/* Supplemental linker fragment for Linux remoteproc resource table.
 * Pass after link.x: rustflags = ["-C", "link-arg=-Tlink.x", "-C", "link-arg=-Tlink-rsc.x"]
 */
SECTIONS
{
  .resource_table : ALIGN(8)
  {
    KEEP(*(.resource_table))
  } > FLASH
}
