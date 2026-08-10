#!/bin/bash
# Apeireth ASI 真生产 startup script (主 23:44 + 主 22:33 + 主 17:33)

set -e

echo "Apeireth ASI 真生产启动 (主 22:33 ASI 北极星)..."

# Check Python
python3 --version

# Install dependencies
pip install -r requirements.txt

# Run tests
echo "Running 真测试 (主 17:43 实事求是)..."
python -m pytest tests/ -q

# Run 真测量
echo "Running 真测量 (主 22:33 ASI 北极星)..."
python -c "
import sys
sys.path.insert(0, '.')
from apeireth.v1002_asi_v02_measure import V1002ASIV02Measure
m = V1002ASIV02Measure()
r = m.measure()
print(f'V0.2 公式实测: total={r.total}, level={r.level}')
"

# Start 真生产 server
echo "Starting 真生产 server (主 22:33 ASI 北极星)..."
python -m apeireth.v1009_web_ui

echo "Apeireth ASI 真生产已启动 (主 22:33 ASI 北极星 + 主 17:43 实事求是)"
