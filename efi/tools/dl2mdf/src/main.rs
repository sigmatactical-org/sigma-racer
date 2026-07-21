//! dl2mdf — convert a stage-1 capture into ASAM MDF4.
//!
//! Input: the `probe-rs run … | tee` log carrying `DL,S` / `DL,T` records
//! (any defmt-print prefix is ignored), plus optionally a `candump -L` log
//! of the bike's bus for the Session-0 side-by-side (see the stage-1 wiring
//! doc in the specs repo).
//!
//! Output: one `.mf4` with a channel group per stream — `sensors`,
//! `trigger`, and `can` when a candump is given — each with a master time
//! channel in seconds. DL time is relative to the first DL record; CAN time
//! is relative to the first frame plus `--can-offset-s` (align by an event
//! visible in both streams, e.g. a throttle blip).
//!
//! Usage:
//!   dl2mdf <capture.log> [--can <candump.log>] [--can-offset-s <f64>] [-o <out.mf4>]

#![forbid(unsafe_code)]

mod args;
mod can_rec;
mod sensor_rec;
mod trigger_rec;
pub(crate) use args::Args;
pub(crate) use can_rec::CanRec;
pub(crate) use sensor_rec::SensorRec;
pub(crate) use trigger_rec::TriggerRec;

use mdf4_rs::{DataType, DecodedValue, MdfWriter};
use std::process::ExitCode;

/// Parse one log line for a `DL,S` record.
fn parse_sensor(line: &str) -> Option<SensorRec> {
    let rest = line.split("DL,S,").nth(1)?;
    let mut parts = rest.trim().split(',');
    let t_us = parts.next()?.parse().ok()?;
    let mut values = [0.0f64; 6];
    for slot in &mut values {
        *slot = parts.next()?.parse().ok()?;
    }
    Some(SensorRec { t_us, values })
}

/// Parse one log line for a `DL,T` record.
fn parse_trigger(line: &str) -> Option<TriggerRec> {
    let rest = line.split("DL,T,").nth(1)?;
    let mut parts = rest.trim().split(',');
    let line_tag = parts.next()?;
    let line = match line_tag {
        "C" => 0,
        "V" => 1,
        _ => return None,
    };
    Some(TriggerRec {
        line,
        count: parts.next()?.parse().ok()?,
        t_us: parts.next()?.parse().ok()?,
        period_us: parts.next()?.parse().ok()?,
        gap_ratio_x100: parts.next()?.parse().ok()?,
    })
}

/// Parse one `candump -L` line: `(1720000000.123456) can0 123#DEADBEEF`.
/// CAN-FD frames (`##`) and error frames are skipped.
fn parse_candump(line: &str) -> Option<CanRec> {
    let line = line.trim();
    let ts = line.strip_prefix('(')?.split(')').next()?;
    let t_s: f64 = ts.parse().ok()?;
    let frame = line.split_whitespace().nth(2)?;
    if frame.contains("##") {
        return None;
    }
    let (id_hex, data_hex) = frame.split_once('#')?;
    let id = u32::from_str_radix(id_hex, 16).ok()?;
    let data_hex: String = data_hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if data_hex.len() % 2 != 0 || data_hex.len() > 16 {
        return None;
    }
    let dlc = (data_hex.len() / 2) as u8;
    let mut data = 0u64;
    for byte in 0..dlc {
        let b = u8::from_str_radix(&data_hex[byte as usize * 2..byte as usize * 2 + 2], 16).ok()?;
        data = (data << 8) | b as u64;
    }
    Some(CanRec { t_s, id, dlc, data })
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut dl_path = None;
    let mut can_path = None;
    let mut can_offset_s = 0.0;
    let mut out_path = None;
    let mut iter = argv.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--can" => can_path = Some(iter.next().ok_or("--can needs a path")?.clone()),
            "--can-offset-s" => {
                can_offset_s = iter
                    .next()
                    .ok_or("--can-offset-s needs a value")?
                    .parse()
                    .map_err(|_| "--can-offset-s: not a number")?;
            }
            "-o" => out_path = Some(iter.next().ok_or("-o needs a path")?.clone()),
            other if !other.starts_with('-') && dl_path.is_none() => {
                dl_path = Some(other.to_string());
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    let dl_path = dl_path.ok_or(
        "usage: dl2mdf <capture.log> [--can <candump.log>] [--can-offset-s <s>] [-o <out.mf4>]",
    )?;
    let out_path = out_path.unwrap_or_else(|| format!("{dl_path}.mf4"));
    Ok(Args {
        dl_path,
        can_path,
        can_offset_s,
        out_path,
    })
}

fn run(args: &Args) -> Result<(), String> {
    let dl_text =
        std::fs::read_to_string(&args.dl_path).map_err(|e| format!("{}: {e}", args.dl_path))?;

    let sensors: Vec<SensorRec> = dl_text.lines().filter_map(parse_sensor).collect();
    let triggers: Vec<TriggerRec> = dl_text.lines().filter_map(parse_trigger).collect();
    if sensors.is_empty() && triggers.is_empty() {
        return Err("no DL,S / DL,T records found in the capture".into());
    }

    // DL time base: seconds relative to the earliest DL record.
    let t0_us = sensors
        .iter()
        .map(|s| s.t_us)
        .chain(triggers.iter().map(|t| t.t_us))
        .min()
        .unwrap_or(0);
    let rel_s = |t_us: u64| (t_us.saturating_sub(t0_us)) as f64 / 1e6;

    let can: Vec<CanRec> = match &args.can_path {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let frames: Vec<CanRec> = text.lines().filter_map(parse_candump).collect();
            if frames.is_empty() {
                return Err(format!("{path}: no candump frames parsed"));
            }
            frames
        }
        None => Vec::new(),
    };

    let mdf = (|| -> mdf4_rs::Result<()> {
        let mut writer = MdfWriter::new(&args.out_path)?;
        writer.init_mdf_file()?;

        // ---- sensors group ----
        if !sensors.is_empty() {
            let cg = writer.add_channel_group(None, |_| {})?;
            let time = writer.add_channel(&cg, None, |ch| {
                ch.data_type = DataType::FloatLE;
                ch.bit_count = 64;
                ch.name = Some("t".into());
            })?;
            writer.set_time_channel(&time)?;
            writer.set_channel_unit(&time, "s")?;

            let names: [(&str, &str); 6] = [
                ("vbatt", "V"),
                ("clt", "degC"),
                ("iat", "degC"),
                ("tps_map", "V"),
                ("an_volt1", "V"),
                ("an_volt2", "V"),
            ];
            let mut prev = time.clone();
            for (name, unit) in names {
                let cn = writer.add_channel(&cg, Some(&prev), |ch| {
                    ch.data_type = DataType::FloatLE;
                    ch.bit_count = 64;
                    ch.name = Some(name.into());
                })?;
                writer.set_channel_unit(&cn, unit)?;
                prev = cn;
            }

            writer.start_data_block_for_cg(&cg, 0)?;
            for rec in &sensors {
                let mut row = vec![DecodedValue::Float(rel_s(rec.t_us))];
                row.extend(rec.values.iter().map(|&x| DecodedValue::Float(x)));
                writer.write_record(&cg, &row)?;
            }
            writer.finish_data_block(&cg)?;
        }

        // ---- trigger group ----
        if !triggers.is_empty() {
            let cg = writer.add_channel_group(None, |_| {})?;
            let time = writer.add_channel(&cg, None, |ch| {
                ch.data_type = DataType::FloatLE;
                ch.bit_count = 64;
                ch.name = Some("t".into());
            })?;
            writer.set_time_channel(&time)?;
            writer.set_channel_unit(&time, "s")?;

            let line = writer.add_channel(&cg, Some(&time), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("line".into());
            })?;
            writer.add_value_to_text_conversion(
                &[(0i64, "CRANK"), (1i64, "CAM")],
                "?",
                Some(&line),
            )?;
            let count = writer.add_channel(&cg, Some(&line), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("count".into());
            })?;
            let period = writer.add_channel(&cg, Some(&count), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("period".into());
            })?;
            writer.set_channel_unit(&period, "us")?;
            writer.add_channel(&cg, Some(&period), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("gap_ratio_x100".into());
            })?;

            writer.start_data_block_for_cg(&cg, 0)?;
            for rec in &triggers {
                writer.write_record(
                    &cg,
                    &[
                        DecodedValue::Float(rel_s(rec.t_us)),
                        DecodedValue::UnsignedInteger(rec.line),
                        DecodedValue::UnsignedInteger(rec.count),
                        DecodedValue::UnsignedInteger(rec.period_us),
                        DecodedValue::UnsignedInteger(rec.gap_ratio_x100),
                    ],
                )?;
            }
            writer.finish_data_block(&cg)?;
        }

        // ---- optional CAN group (Session-0 side-by-side) ----
        if !can.is_empty() {
            let can_t0 = can.first().map(|c| c.t_s).unwrap_or(0.0);
            let cg = writer.add_channel_group(None, |_| {})?;
            let time = writer.add_channel(&cg, None, |ch| {
                ch.data_type = DataType::FloatLE;
                ch.bit_count = 64;
                ch.name = Some("t".into());
            })?;
            writer.set_time_channel(&time)?;
            writer.set_channel_unit(&time, "s")?;
            let id = writer.add_channel(&cg, Some(&time), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("can_id".into());
            })?;
            let dlc = writer.add_channel(&cg, Some(&id), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("dlc".into());
            })?;
            writer.add_channel(&cg, Some(&dlc), |ch| {
                ch.data_type = DataType::UnsignedIntegerLE;
                ch.name = Some("data_be".into());
            })?;

            writer.start_data_block_for_cg(&cg, 0)?;
            for frame in &can {
                writer.write_record(
                    &cg,
                    &[
                        DecodedValue::Float(frame.t_s - can_t0 + args.can_offset_s),
                        DecodedValue::UnsignedInteger(frame.id as u64),
                        DecodedValue::UnsignedInteger(frame.dlc as u64),
                        DecodedValue::UnsignedInteger(frame.data),
                    ],
                )?;
            }
            writer.finish_data_block(&cg)?;
        }

        writer.finalize()
    })();
    mdf.map_err(|e| format!("mdf write failed: {e:?}"))?;

    eprintln!(
        "{}: {} sensor, {} trigger, {} can records",
        args.out_path,
        sensors.len(),
        triggers.len(),
        can.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(args) => args,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("dl2mdf: {msg}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sensor_record_with_defmt_prefix() {
        let line = "0.123456 INFO  DL,S,1234567,12.61,84.2,31.5,0.84,2.51,0.00";
        let rec = parse_sensor(line).unwrap();
        assert_eq!(rec.t_us, 1_234_567);
        assert!((rec.values[0] - 12.61).abs() < 1e-9);
        assert!((rec.values[5] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn parses_trigger_record_both_lines() {
        let crank = parse_trigger("INFO DL,T,C,42,1000500,4545,100").unwrap();
        assert_eq!(crank.line, 0);
        assert_eq!(crank.count, 42);
        assert_eq!(crank.t_us, 1_000_500);
        assert_eq!(crank.period_us, 4_545);
        assert_eq!(crank.gap_ratio_x100, 100);
        let cam = parse_trigger("DL,T,V,7,2000000,60000,0").unwrap();
        assert_eq!(cam.line, 1);
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert!(parse_sensor("INFO stage 1: characterization data logger").is_none());
        assert!(parse_trigger("WARN DL,X,dropped_edges,3").is_none());
        assert!(parse_sensor("DL,T,C,1,2,3,4").is_none());
    }

    #[test]
    fn parses_candump_classic_frame() {
        let rec = parse_candump("(1720000000.123456) can0 23A#DEADBEEF01").unwrap();
        assert!((rec.t_s - 1_720_000_000.123456).abs() < 1e-6);
        assert_eq!(rec.id, 0x23A);
        assert_eq!(rec.dlc, 5);
        assert_eq!(rec.data, 0xDEAD_BEEF_01);
    }

    #[test]
    fn skips_fd_and_malformed_candump_lines() {
        assert!(parse_candump("(1.0) can0 123##1DEADBEEF").is_none());
        assert!(parse_candump("garbage").is_none());
        assert!(parse_candump("(1.0) can0 123#DEADBEE").is_none()); // odd nibbles
    }

    #[test]
    fn end_to_end_writes_readable_mf4() {
        let dir = std::env::temp_dir().join("dl2mdf-test");
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("capture.log");
        let out = dir.join("capture.mf4");
        std::fs::write(
            &log,
            "INFO DL,S,1000,12.5,80.0,30.0,0.8,2.5,0.0\n\
             INFO DL,S,11000,12.6,80.1,30.0,0.8,2.5,0.0\n\
             INFO DL,T,C,1,1500,4545,100\n\
             INFO DL,T,C,2,6045,4545,100\n",
        )
        .unwrap();
        let args = Args {
            dl_path: log.to_string_lossy().into_owned(),
            can_path: None,
            can_offset_s: 0.0,
            out_path: out.to_string_lossy().into_owned(),
        };
        run(&args).unwrap();

        let mdf = mdf4_rs::MDF::from_file(out.to_str().unwrap()).unwrap();
        assert_eq!(mdf.channel_groups().len(), 2);
        let sensors = &mdf.channel_groups()[0];
        assert_eq!(sensors.channels().len(), 7);
        let values = sensors.channels()[1].values().unwrap();
        assert_eq!(values.len(), 2);
    }
}
