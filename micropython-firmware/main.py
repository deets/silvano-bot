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

    def __init__(self):
        self._bus = I2C(0, scl=Pin(14), sda=Pin(13), freq=100000)
        self._command = None
        self._last_command = time.time()
        self.drive(128, 128)

    def drive(self, left, right):
        #left, right = int(-left * 127.0 + 128), int(-right * 127.0 + 128)
        self._bus.writeto_mem(self.ADDRESS, self.MOTOR_LEFT, bytes([left]))
        self._bus.writeto_mem(self.ADDRESS, self.MOTOR_RIGHT, bytes([right]))

    async def motor_task(self):
        while True:
            if self._command is not None:
                left, right = self._command
                self._command = None
                self._last_command = time.time()
                self.drive(left, right)
            elif time.time() - self._last_command > 0.5:
                self.drive(128, 128)
            await asyncio.sleep_ms(5)

    def set_command(self, left, right):
        self._command = (left << 1, right << 1)

INDEX_HTML_HEADER = """HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n"""
MOVE_RESPONSE = """HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nCache-Control: no-cache\r\n\r\n{}"""
ERROR_RESPONSE = """HTTP/1.0 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n"""


def parse_arg(arg):
    number = arg.split("=")[1]
    return float(number)


def process_move_request(request, set_command):
    args = request.decode("ascii").split("?")[1].split("HTTP")[0].strip()
    left, right = args.split("&")
    left, right = parse_arg(left), parse_arg(right)
    set_command(left, right)

async def handler(set_command, reader: asyncio.stream.StreamReader, writer: asyncio.StreamWriter):
    """
    Async handler function to handle new connections.
    """
    while True:
        while True:
            left = (await reader.read(1))[0]
            if left & 0x80:
                left = left & 0x7f
                break
        right = (await reader.read(1))[0]
        set_command(left, right)

async def main():
    loop = asyncio.get_event_loop()
    md23 = MD23()
    server = await asyncio.start_server(lambda r, w: handler(md23.set_command, r, w), "0.0.0.0", 80)
    loop.create_task(md23.motor_task())  # Create a task to run the main function
    loop.run_forever()

asyncio.run(main())
