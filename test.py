import pyvector
import time
import asyncio
import pathlib
import json
import random
import time

contents = pathlib.Path("vector_config_1.toml").read_text()
instance = pyvector.Vector(contents)


async def main():
    iterations = 100
    batch_size = 50
    total_size = iterations * batch_size
    start = time.time()
    for i in range(iterations):
        batch = [json.dumps({"batch": i, "item": idx}).encode() for idx in range(0, batch_size)]
        await instance.send_batch(
            "in",
            batch
        )
    end = time.time()
    taken = (end - start)
    print("Time taken: ", taken)
    print("Per iteration: ", taken / iterations)
    print("Per item: ", taken / total_size)
    await asyncio.sleep(1)
    print("Stopping")
    instance.get_metrics()
    await instance.stop()
    print("Stopped!")


# print(instance.is_running())
# instance.start()
# print(instance.is_running())
# instance.stop()
# print(instance.is_running())
# print(instance.is_running())
time.sleep(5)
# print(instance.is_running())

print(instance)
asyncio.run(main())
