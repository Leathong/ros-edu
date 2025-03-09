OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
KERNEL_SPACE_OFFSET = 0xffffffc000000000;
BOOT_SIZE = 8K;

KERNEL_START_ADDR = KERNEL_SPACE_OFFSET + BASE_ADDRESS + BOOT_SIZE;

MEMORY {
    BOOT (rwx) : ORIGIN = BASE_ADDRESS, LENGTH = BOOT_SIZE
    FLASH (rwx) : ORIGIN = BASE_ADDRESS + BOOT_SIZE, LENGTH = 16M
    RAM (rwx)  : ORIGIN = KERNEL_START_ADDR, LENGTH = 16M
}

SECTIONS
{
    . = BASE_ADDRESS;
    sentry = .;
    .text.entry : {
        *(.text.entry)
        . = ALIGN(4K);
    } AT>BOOT 
    eentry = .;

    . = KERNEL_START_ADDR;
    skernel = .;
    .text : {
        stext = .;
        *(.text .text.*)
        . = ALIGN(4K);
        etext = .;
    } >RAM AT>FLASH 
    
    .rodata : {
        srodata = .;
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        . = ALIGN(4K);
        erodata = .;
    } >RAM AT>FLASH 

    .data : {
        sdata = .;
        *(.data .data.*)
        *(.sdata .sdata.*)
        . = ALIGN(4K);
        edata = .;
    } >RAM AT>FLASH 

    .bss : {
        sbss_with_stack = .;
        *(.bss.stack)
        . = ALIGN(4K);
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        . = ALIGN(4K);
        ebss = .;
    } >RAM AT>FLASH 
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}