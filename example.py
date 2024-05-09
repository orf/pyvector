import uuid
import pyvector
import asyncio
import json

config_local = """
[sources.python]
type = "python"

[sinks.file]
type = "file"
inputs = ["python"]
path = "/tmp/output.txt"
encoding.codec = "json"
"""

config_aws = """
[sources.python]
type = "python"

[sinks.s3]
type = "aws_s3"
inputs = ["python"]
bucket = "my-bucket"
encoding.codec = "json"

[sinks.sqs]
type = "aws_sqs"
inputs = ["python"]
queue_url = "..."
encoding.codec = "json"
"""


async def send_to_vector():
    vector = pyvector.Vector(config_local)
    await vector.start()

    for i in range(1_000_000):
        data = json.dumps({"i": i, "uuid": str(uuid.uuid4())}).encode()
        await vector.send("python", data)

    await vector.stop()

asyncio.run(send_to_vector())