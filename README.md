# pyvector-rs

An _experiment_ to integrate the power of [Vector](https://vector.dev/) with Python!

## Sending messages _reliably_ can be quite hard

Even with something simple like SQS, when sending a batch of messages individual messages can fail while the rest 
succeed. So you need to detect this (and other errors), keep them in memory, and retry them with some kind of backoff. 
But what if your process is asked to exist before these have been sent successfully? What do you do? And how do you 
handle a large spike in send failures? You don't want messages to pile up and exhaust your memory, which would result in 
you loosing all your messages. So you need some kind of disk buffer. And you need metrics around this, and logging, and 
the rest.

If you squint a bit, this begins to look a lot like [Vector](https://vector.dev/).

## What does this do?

This library integrates Vector with Python (without using an external process) and provides a custom `python` source 
that allows you to send Python `bytes` to Vector with minimal copying.

You can then use any [of the many available sinks](https://vector.dev/docs/reference/configuration/sinks/) to forward
this data anywhere, with Vector handling all the complexities around batching, buffering to disk or memory, retries, 
rate-limiting, partitioning, authentication, backpressure and more.

The code below sends 1 million events to a SQS queue, a S3 bucket and an Elasticsearch cluster:

```python
import uuid
import pyvector
import asyncio
import json

# Vector config: https://vector.dev/docs/reference/configuration/
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
        await vector.send(source="python", data=data)
    
    await vector.stop()

asyncio.run(send_to_vector())
```
