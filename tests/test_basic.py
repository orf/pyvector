import json

import pytest
import pyvector
import textwrap


@pytest.mark.asyncio
async def test_basic(tmp_path):
    directory = tmp_path / "output"
    directory.mkdir()
    output_file = tmp_path / "output.txt"

    config = f"""
    [sources.python]
    type = "python"
    
    [sinks.file]
    type = "file"
    inputs = ["python"]
    path = "{output_file}"
    encoding.codec = "json"
    """

    vector = pyvector.Vector(textwrap.dedent(config))
    await vector.start()
    data = json.dumps({"hi": 123}).encode()
    await vector.send("python", data)
    await vector.stop()
    file_contents = json.loads(output_file.read_text())
    assert file_contents['hi'] == 123
