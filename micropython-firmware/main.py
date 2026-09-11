import network
import time
from machine import Pin, I2C
import asyncio
import os

ap = network.WLAN(network.WLAN.IF_AP) # create access-point interface
ap.config(ssid='esp-radio')              # set the SSID of the access point
ap.config(max_clients=10)             # set how many clients can connect to the network
ap.active(True)
print("HALLO DIEZ", ap.ifconfig())

class MD23:

    ADDRESS = 0x58
    MOTOR_LEFT = 1
    MOTOR_RIGHT = 0

    def __init__(self, bus):
        self._bus = bus

    def drive(self, left, right):
        self._bus.writeto_mem(self.ADDRESS, self.MOTOR_LEFT, bytes([left]))
        self._bus.writeto_mem(self.ADDRESS, self.MOTOR_RIGHT, bytes([right]))

LEFT, RIGHT = 0.0, 0.0

async def motor_driver():
    i2c = I2C(0)
    i2c = I2C(1, scl=Pin(14), sda=Pin(13), freq=100000)
    md23 = MD23(i2c)
    while True:
        try:
            left, right = int(-LEFT * 127.0 + 128), int(-RIGHT * 127.0 + 128)
            md23.drive(left, right)
        except Exception as e:
            print("i2c error", e)
        await asyncio.sleep_ms(5)


INDEX_HTML_HEADER = """HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n"""
MOVE_RESPONSE = """HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: 0\r\nCache-Control: no-cache\r\n\r\n"""
ERROR_RESPONSE = """HTTP/1.0 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n"""


def parse_arg(arg):
    number = arg.split("=")[1]
    return float(number)


def process_move_request(request):
    global LEFT, RIGHT
    args = request.decode("ascii").split("?")[1].split("HTTP")[0].strip()
    left, right = args.split("&")
    LEFT, RIGHT = parse_arg(left), parse_arg(right)


async def handler(reader: asyncio.stream.StreamReader, writer: asyncio.StreamWriter):
    """
    Async handler function to handle new connections.
    """
    request = await reader.readline()
    while True:
        header = await reader.readline()
        if not header.strip():
            break
    if request.startswith("GET / "):
        with open("index.html") as inf:
            writer.write(INDEX_HTML_HEADER.format(os.stat("index.html")[6]))
            await writer.drain()
            while True:
                block = inf.read(1024)
                if not block:
                    break
                writer.write(block)
                await writer.drain()
    elif request.startswith("GET /move?"):
        process_move_request(request)
        writer.write(MOVE_RESPONSE)
        await writer.drain()
    else:
        writer.write(ERROR_RESPONSE)
        await writer.drain()
    writer.close()
    await writer.wait_closed()

async def main():
    loop = asyncio.get_event_loop()
    server = await asyncio.start_server(handler, "0.0.0.0", 80)
    loop.create_task(motor_driver())  # Create a task to run the main function
    loop.run_forever()

asyncio.run(main())
