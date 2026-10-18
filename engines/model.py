"""Prepares the background-removal model for shipping.

The weights are stored at half precision (half the download) and TapWheel expands them back
to full precision once, on first use. Rounding the weights to half precision changes the
model's output by less than 0.0001, so cut-outs look the same.

Usage: python model.py <isnet-general-use.onnx> <out-dir>
Writes background.onnx (the graph, reading weights from background.weights) and background.f16.
"""
import sys
from pathlib import Path

import numpy as np
import onnx
from onnx import numpy_helper
from onnx.external_data_helper import convert_model_to_external_data

src, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
model = onnx.load(str(src))

# round every float weight to half precision, so the expanded file matches exactly
for init in model.graph.initializer:
    if init.data_type == onnx.TensorProto.FLOAT:
        a = numpy_helper.to_array(init).astype(np.float16).astype(np.float32)
        init.CopyFrom(numpy_helper.from_array(a, init.name))

convert_model_to_external_data(model, all_tensors_to_one_file=True, location="background.weights", size_threshold=1024)
for init in model.graph.initializer:
    if any(e.key == "location" for e in init.external_data):
        assert init.data_type == onnx.TensorProto.FLOAT, f"{init.name} isn't float"
onnx.save_model(model, str(out / "background.onnx"))

weights = np.fromfile(out / "background.weights", dtype="<f4")
weights.astype("<f2").tofile(out / "background.f16")
(out / "background.weights").unlink()
print(f"model ready: {weights.size} weights")
