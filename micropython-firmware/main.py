import network
import time
from machine import Pin, I2C

ap = network.WLAN(network.WLAN.IF_AP) # create access-point interface
ap.config(ssid='esp-radio')              # set the SSID of the access point
ap.config(max_clients=10)             # set how many clients can connect to the network
ap.active(True)

class MD23:

    ADDRESS = 0x58
    MOTOR_LEFT = 0
    MOTOR_RIGHT = 1

    def __init__(self, bus):
        self._bus = bus

    def drive(self, left, right):
        self._bus.writeto_mem(self.ADDRESS, self.MOTOR_LEFT, bytes([left]))
        time.sleep_ms(5)
        self._bus.writeto_mem(self.ADDRESS, self.MOTOR_RIGHT, bytes([right]))
        time.sleep_ms(5)

i2c = I2C(0)
i2c = I2C(1, scl=Pin(14), sda=Pin(13), freq=100000)

md23 = MD23(i2c)

speed = 0
while True:
    try:
        md23.drive(speed, speed)
    except Exception as e:
        print("i2c error", e)
    speed = (speed + 1) % 256
    time.sleep_ms(5)
