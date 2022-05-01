all: 
	# rustup target add riscv64gc-unknown-none-elf
	cd codes/user && make elf
	cd codes/os && make release BOARD=k210

env:
	rustup update
	cargo install cargo-binutils
	cd codes/fat32-fuse && sh qemu_fs.sh
	cd codes/os && make env

run:
	cd codes/user && make elf
	cd codes/os && make fat32
	cd codes/os && make run