/* Memory layout of the bl602 microcontroller */
/* 1K = 1 KiBi = 1024 bytes */
MEMORY
{
  RAM : ORIGIN = 0x20080000, LENGTH = 272K
  FLASH : ORIGIN = 0x23000000, LENGTH = 2M
}

ENTRY(reset);

EXTERN(RESET_VECTOR);

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    /* First entry: initial Stack Pointer value */
    LONG(ORIGIN(RAM) + LENGTH(RAM));

    /* Second entry: reset vector */
    KEEP(*(.vector_table.reset_vector));
  } > FLASH

  .text :
  {
    *(.text .text.*);
  } > FLASH
}
