/* i.MX8M Plus Cortex-M7 — DDR carve-out (default).
 *
 * `build.rs` selects this file unless the `memory-itcm` feature is enabled.
 * See `memory-ddr.x` (identical) and `docs/M7_BRINGUP.md`.
 */
MEMORY
{
  FLASH (rx)  : ORIGIN = 0x80000000, LENGTH = 2M
  RAM   (rwx) : ORIGIN = 0x80200000, LENGTH = 2M
}
