# Makefile for syscall_tracer.cpp using casacore from casatables_impl

# Compiler settings
CXX = g++
CXXFLAGS = -std=c++17 -O2 -Wall -Wextra

# Casacore paths (relative to casatables_impl directory)
CASACORE_ROOT = ../casatables_impl/casacore
CASACORE_INCLUDE = $(CASACORE_ROOT)/casa
CASACORE_TABLES_INCLUDE = $(CASACORE_ROOT)/tables

# Include directories
INCLUDES = -I$(CASACORE_INCLUDE) \
           -I$(CASACORE_TABLES_INCLUDE) \
           -I$(CASACORE_ROOT)

# Target executable
TARGET = syscall_tracer_cpp

# Source files
SRCS = syscall_tracer.cpp

# Build target
$(TARGET): $(SRCS)
	@echo "Compiling $(TARGET) with casacore..."
	$(CXX) $(CXXFLAGS) $(INCLUDES) $(SRCS) -o $(TARGET)
	@echo "Build complete: $(TARGET)"

# Clean target
clean:
	rm -f $(TARGET)

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
