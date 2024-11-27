TARGET := mapfile

CC := riscv64-unknown-linux-musl-gcc
STRIP := riscv64-unknown-linux-musl-strip

all: $(TARGET)

%: %.c
	$(CC) -static $< -o $@
	$(STRIP) $@

clean:
	@rm -rf ./$(TARGET)
