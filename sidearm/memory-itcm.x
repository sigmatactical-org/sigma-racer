/* i.MX8M Plus Cortex-M7 — ITCM code + DTCM data (U-Boot bootaux @ 0x0000_0000).
 *
 * Matches NXP MCUXpresso `MIMX8ML8xxxxx_cm7_ram.ld`. Use for early bring-up
 * when the image fits in on-chip TCM (128 KiB ITCM + 128 KiB DTCM). Copy the
 * flat binary to address 0 and `bootaux 0` (or the ELF entry) from U-Boot.
 */
MEMORY
{
  FLASH (rx)  : ORIGIN = 0x00000000, LENGTH = 128K
  RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 128K
}
