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
    .text.entry : {
        *(.text.entry)
        . = ALIGN(4K);
    } AT>BOOT 

    . = KERNEL_START_ADDR;
    skernel = .;
    stext = .;
    .text : {
        strampoline = .;
        *(.text .text.*)
    } >RAM AT>FLASH 
    . = ALIGN(4K);
    etext = .;
    
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } >RAM AT>FLASH 

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } >RAM AT>FLASH 

    . = ALIGN(4K);
    edata = .;
    sbss_with_stack = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    } >RAM AT>FLASH 

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}