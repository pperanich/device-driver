# Build the SV target for examples/sv_e2e/blinky.ddsl and run the
# Verilator co-simulation harness against it.
#
# Requires: a built `ddc` CLI binary (cargo build -p device-driver-cli)
# and a verilator install (>= 5.0).
#
# Usage:
#   make            # generate SV + build + run
#   make generate   # generate SV only
#   make clean      # remove generated files

REPO     := $(realpath $(CURDIR)/../..)
DDC      := $(REPO)/target/debug/ddc
GEN_DIR  := generated
SV_FILES := $(GEN_DIR)/blinky.sv

.PHONY: all generate sim clean

all: sim

$(DDC):
	cd $(REPO) && cargo build -p device-driver-cli

$(SV_FILES): blinky.ddsl $(DDC)
	$(DDC) build -t sv -o $(GEN_DIR) blinky.ddsl

generate: $(SV_FILES)

sim: $(SV_FILES) tb_blinky.sv
	verilator --binary -Wno-fatal -sv \
	  --top-module tb_blinky \
	  $(GEN_DIR)/blinky.sv \
	  tb_blinky.sv \
	  -o Vtb_blinky
	./obj_dir/Vtb_blinky

clean:
	rm -rf $(GEN_DIR) obj_dir
