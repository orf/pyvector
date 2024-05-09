# pyvector-rs

Combine the power of [Vector](https://vector.dev/) with Python! This is very WIP right now

## What is this?

This library integrates Vector with Python (without using an external process), and provides a custom `python` source 
that allows you to send in-memory data to Vector with minimal copying. You can use any 
[of the many available sinks](https://vector.dev/docs/reference/configuration/sinks/) to send this data anywhere, with 
Vector handling all the complexities around batching, buffering to disk or memory, retries, rate-limiting, partitioning,
authentication, backpressure and more.

The code below sends 1 million events to a SQS queue, a S3 bucket and an Elasticsearch cluster:

```python
import uuid
import pyvector
import asyncio
import json

config = """
[sources.python]
type = "python"

[sinks.s3]
type = "aws_s3"
inputs = ["python"]
bucket = "my-bucket"

[sinks.sqs]
type = "aws_sqs"
inputs = ["python"]
queue_url = "..."

[sinks.elasticsearch]
type = "elasticsearch"
inputs = ["python"]
endpoints = ["..."]
"""

async def send_to_vector():
    vector = pyvector.Vector(config)
    await vector.start()

    for i in range(1_000_000):
        data = json.dumps({"i": i, "uuid": str(uuid.uuid4())}).encode()
        await vector.send("python", data)
    
    await vector.stop()

asyncio.run(send_to_vector())
```
