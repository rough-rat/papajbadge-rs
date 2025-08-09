OPENOCD_BIN := ./openocd/openocd
OPENOCD_CFG := ./openocd/wch-riscv.cfg
GDB := rust-gdb
OPENOCD_PORT := 3333

.PHONY: debug
debug:
	$(GDB) $(BIN_PATH) \
		-ex "set pagination off" \
		-ex "target extended-remote :3333" \
		-ex "set remotetimeout 5000" \
		-ex "monitor reset halt" \
		-ex "load" \
		 2> /dev/null 

# I get a lot of errors like "(Internal error: pc 0x0 in read in CU, but not in symtab.)"
# For now i pipe stderr to /dev/null to avoid cluttering the output.

# note - this supresses some other useful warnings

.PHONY: attach
attach:
	$(GDB) $(BIN_PATH) \
		-ex "set pagination off" \
		-ex "target extended-remote :3333" \
		-ex "set remotetimeout 5000" \
		-ex "monitor reset halt" \
		 2> /dev/null 

.PHONY: spawn-openocd
spawn-openocd:
	$(OPENOCD_BIN) -f $(OPENOCD_CFG)
	pkill -f $(OPENOCD_BIN)

unlock-target:
	wchisp config unprotect
	wchisp config enable-debug

reset-target:
	wchisp config reset