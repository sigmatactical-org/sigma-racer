/* i.MX8M Plus Cortex-M7 — DDR carve-out (remoteproc / U-Boot bootaux @ 0x8000_0000).
 *
 * Matches NXP MCUXpresso `MIMX8ML8xxxxx_cm7_ddr_ram.ld` and Zephyr
 * `verdin_imx8mp_mimx8ml8_m7_ddr.dts`. The A53 loader copies the image into
 * reserved DDR; LMA must equal VMA for Linux remoteproc (no AT> load regions).
 *
 * Align the Linux `reserved-memory` node and U-Boot `bootaux` address with
 * these origins before running on hardware.
 */
MEMORY
{
  FLASH (rx)  : ORIGIN = 0x80000000, LENGTH = 2M
  RAM   (rwx) : ORIGIN = 0x80200000, LENGTH = 2M
}
