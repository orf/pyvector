import pathlib
import uuid

import pyvector
import time
import asyncio
import json
import random
import datetime
import time

config_one = """
[sources.in_one]
type = "python"

[sinks.out]
type = "file"
inputs = ["in_one"]
path = "test_outputs/one.txt"
encoding.codec = "json"
"""

config_two = """
[sources.in_two]
type = "python"

[sinks.out]
type = "file"
inputs = ["in_two"]
path = "test_outputs/two.txt"
encoding.codec = "json"
"""

instance_1 = pyvector.Vector(config_one)
instance_2 = pyvector.Vector(config_two)
instance_integration = pyvector.Vector(pathlib.Path("integration.toml").read_text())


async def main():
    await instance_integration.start()
    iterations = 1_000
    contents = [
        json.dumps({"instance": str(uuid.uuid4())}).encode()
        for _ in range(iterations)
    ]
    total_bytes = sum(len(b) for b in contents)

    futures = [
        instance_integration.send("in", c)
        for c in contents
    ]
    start = datetime.datetime.now()
    for fut in futures:
        await fut
    # await asyncio.gather(*futures)
    time_taken = datetime.datetime.now() - start
    time_taken_seconds = time_taken / datetime.timedelta(seconds=1)
    throughput = total_bytes / time_taken_seconds
    print(f"Time taken: {time_taken}")
    print(f"Throughput: {throughput} bytes per second")
    print(f"Total bytes: {total_bytes}")
    # await asyncio.sleep(4)
    await instance_integration.stop()

async def main4():
    await instance_1.start()
    contents = json.dumps({"instance": 1}).encode()
    iterations = 1_000
    total_bytes = len(contents) * iterations

    futures = [
        instance_1.send("in_one", contents)
        for _ in range(iterations)
    ]
    start = datetime.datetime.now()
    for fut in futures:
        await fut
    # await asyncio.gather(*futures)
    time_taken = datetime.datetime.now() - start
    time_taken_seconds = time_taken / datetime.timedelta(seconds=1)
    throughput = total_bytes / time_taken_seconds
    print(f"Time taken: {time_taken}")
    print(f"Throughput: {throughput} bytes per second")
    print(f"Total bytes: {total_bytes}")
    # await asyncio.sleep(4)
    await instance_1.stop()


async def main3():
    await instance_1.start()
    await instance_2.start()
    print("Started!")
    await instance_1.send("in_one", json.dumps({"instance": 1}).encode())
    await instance_2.send("in_two", json.dumps({"instance": 2}).encode())
    print("Sent!")
    await asyncio.sleep(1)
    await instance_1.stop()
    await instance_2.stop()
    print("Stopped!")


async def main2():
    iterations = 100
    batch_size = 50
    total_size = iterations * batch_size
    start = time.time()
    for i in range(iterations):
        batch = [json.dumps({"batch": i, "hi": True, "item": idx}).encode() for idx in range(0, batch_size)]
        await instance.send_batch(
            "in",
            batch
        )
    end = time.time()
    taken = (end - start)
    print("Time taken: ", taken)
    print("Per iteration: ", taken / iterations)
    print("Per item: ", taken / total_size)
    await asyncio.sleep(5)
    instance.get_metrics()
    print("Stopping")
    await instance.stop()
    print("Stopped!")


asyncio.run(main())
