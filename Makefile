OUTDIR=target/riscv64gc-unknown-none-elf/debug

default: run

build_user:
	@cd user && cargo build && cd ../easy_fs_fuse && cargo run -- --source=../user/target/riscv64gc-unknown-none-elf/debug/ --target=target/

build:
	@cargo build

copy_bin: build
	@rust-objcopy --strip-all $(OUTDIR)/os -O binary $(OUTDIR)/os.bin

run: copy_bin build_user
	@qemu-system-riscv64 \
	-d page,cpu_reset,guest_errors \
	-D qemu.log \
	-machine virt \
	-nographic \
	-bios os/bootloader/rustsbi-qemu.bin \
	-device loader,file=$(OUTDIR)/os.bin,addr=0x80200000 \
	-global virtio-mmio.force-legacy=false \
	-drive file=easy_fs_fuse/target/fs.img,if=none,format=raw,id=x0 \
	-device virtio-blk-device,drive=x0