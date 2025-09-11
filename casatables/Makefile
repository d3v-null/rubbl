# Makefile for syscall_tracer.cpp using casacore from casatables_impl

# Compiler settings (prefer Homebrew GCC over clang on macOS)
CXX := $(shell command -v g++-14 || command -v g++-13 || command -v g++-12 || command -v g++)
CXXFLAGS = -std=c++17 -O0 -g -ggdb -Wall -Wextra -Wno-deprecated-declarations -Dcasacore=rubbl_casacore -DUSE_THREADS=1

# macOS: match deployment target of vendored objects to silence link warnings
OS_NAME := $(shell uname -s)
ifeq ($(OS_NAME),Darwin)
MACOSX_DEPLOYMENT_TARGET ?= 15.5
CXXFLAGS += -mmacosx-version-min=$(MACOSX_DEPLOYMENT_TARGET)
LDFLAGS += -mmacosx-version-min=$(MACOSX_DEPLOYMENT_TARGET)
export MACOSX_DEPLOYMENT_TARGET
endif

# Casacore paths (relative to casatables_impl directory)
CASACORE_ROOT = ../casatables_impl/casacore

# Link against the static library produced by rubbl_casatables_impl build.rs
RUBBL_ROOT := $(abspath ..)
DEBUG_LIB := $(RUBBL_ROOT)/target/debug/build/rubbl_casatables_impl-*/out/libcasatables_impl.a
RELEASE_LIB := $(RUBBL_ROOT)/target/release/build/rubbl_casatables_impl-*/out/libcasatables_impl.a
CASA_IMPL_LIB := $(firstword $(wildcard $(DEBUG_LIB)) $(wildcard $(RELEASE_LIB)))

# Include directories - need to include the parent directory so <casacore/casa/aips.h> works
INCLUDES = -I../casatables_impl

# Target executable
TARGET = syscall_tracer

# Source files
SRCS = syscall_tracer.cpp

# Extra casacore sources needed to satisfy unresolved symbols when linking only the subset from libcasatables_impl.a
EXTRA_SRCS = TableUtil_stubs.cc
EXTRA_OBJS = $(notdir $(EXTRA_SRCS:.cc=.o))

# Build target
$(TARGET): $(SRCS) $(EXTRA_OBJS)
	@echo "Compiling $(TARGET) with casacore..."
	@if [ -z "$(CASA_IMPL_LIB)" ]; then \
		echo "Building rubbl_casatables_impl to produce libcasatables_impl.a..."; \
		(cd $(RUBBL_ROOT) && cargo build -p rubbl_casatables_impl >/dev/null); \
	fi
	@LIB_PATH=$(firstword $(wildcard $(DEBUG_LIB)) $(wildcard $(RELEASE_LIB))); \
	if [ -z "$$LIB_PATH" ]; then \
		echo "Error: libcasatables_impl.a not found in target. Aborting."; \
		exit 1; \
	fi; \
	echo "Linking with $$LIB_PATH"; \
	$(CXX) $(CXXFLAGS) $(INCLUDES) $(SRCS) $(EXTRA_OBJS) $$LIB_PATH $(LDFLAGS) -o $(TARGET)
	@echo "Build complete: $(TARGET)"

$(EXTRA_OBJS): %.o: %.cc
	$(CXX) -c $(CXXFLAGS) $(INCLUDES) $< -o $@

# Clean target
clean:
	rm -f $(TARGET) $(EXTRA_OBJS)

# Help target
help:
	@echo "Available targets:"
	@echo "  make          - Build the syscall tracer"
	@echo "  make clean    - Clean build artifacts"
	@echo "  make help     - Show this help"
	@echo ""
	@echo "This Makefile compiles syscall_tracer.cpp using casacore"
	@echo "from the casatables_impl directory."

.PHONY: clean help
