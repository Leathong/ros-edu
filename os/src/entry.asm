    .equ PERMISSION_XRWV, 0b1111
    .section .text.entry
    .globl _start
_start:
    # Setup stack
    la t0, stack_addr
    ld t0, 0(t0)
    mv sp, t0

    call _init_page_table

    # load main function address
    la t0, ros_main_addr
    ld t0, 0(t0)

    li s0, 0
    # jump to main
    jalr ra, t0, 0 

_init_page_table:
    # load page table
    la t0, tmp_page_table
    srli t0, t0, 12      
    li t1, (0x8 << 60)   
    or t0, t0, t1   
    csrw satp, t0
    sfence.vma
    ret
stack_addr:
    .dword boot_stack_top
ros_main_addr:
    .dword ros_main
    .align 12
tmp_page_table:
    .set n, 0
    .rept 256
        .dword (n << 28) | PERMISSION_XRWV
        .set n, n + 1
    .endr
    .set n, 0
    .rept 256
        .dword (n << 28) | PERMISSION_XRWV
        .set n, n + 1
    .endr

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:

