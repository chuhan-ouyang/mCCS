#!/bin/bash
# run_service_and_bench.sh
#
# This script starts the MCCS service and then launches the all-reduce benchmark.
# It takes one argument (nodeid) and ensures that the MCCS service command finishes
# before starting the allreduce benchmark.

if [ -z "$1" ]; then
    echo "Usage: $0 <nodeid>"
    exit 1
fi

NODEID="$1"

echo "Starting MCCS service on nodeid: $NODEID..."
just back "$NODEID" &
SERVICE_PID=$!

# Wait for the MCCS service command to complete before continuing
echo "Waiting for MCCS service (PID $SERVICE_PID) to finish..."

echo "MCCS service has completed. Launching All-Reduce Benchmark on nodeid: $NODEID..."
just allreduce_bench "$NODEID" 2 2>&1 | tee allreduce_bench.log
