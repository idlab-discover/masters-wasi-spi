# run wasmtime in pico 2: blinky

run build.sh


## pico 2 pinout

BME280 Pin,Function,Pico 2 Pin
+,3.3V Power,3V3 (Pin 36)
-,Ground,GND (Pin 38)
C,SCK (Clock),GP18 (Pin 24)
D,MOSI (Data In),GP19 (Pin 25)
SDO,MISO (Data Out),GP16 (Pin 21)
CS,CS (Chip Select),GP17 (Pin 22)

PmodOLED Pin,Function,Pico 2 Pin Recommendation,Pico 2 Hardware Peripheral
1,CS (Chip Select),GP13 (Pin 17),SPI1 CSn
2,SDIN (MOSI),GP11 (Pin 15),SPI1 TX
3,Unused,Not Connected,-
4,SCLK (Clock),GP10 (Pin 14),SPI1 SCK
"5, 11",GND,GND (Pin 18 or any GND),Ground
"6, 12",VCC,3V3 OUT (Pin 36),3.3V Power
7,D/C (Data/Cmd),GP2 (Pin 4),Standard GPIO
8,RES (Reset),GP3 (Pin 5),Standard GPIO
9,VBATC (Bat Volt),GP4 (Pin 6),Standard GPIO
10,VDDC (Logic Volt),GP5 (Pin 7),Standard GPIO

## Raspberry pi 4 pinout

```
Raspberry Pi 4 GPIO Header
                                   (Top View)

   (Sensor +)      3.3V Power [ 1] o o [ 2] 5V Power
               SDA / GPIO 2   [ 3] o o [ 4] 5V Power
               SCL / GPIO 3   [ 5] o o [ 6] Ground        (Sensor -)
                     GPIO 4   [ 7] o o [ 8] GPIO 14 (TXD)
                     Ground   [ 9] o o [10] GPIO 15 (RXD)
    (OLED DC)       GPIO 17   [11] o o [12] GPIO 18
   (OLED RES)       GPIO 27   [13] o o [14] Ground        (OLED GND)
 (OLED VBATC)       GPIO 22   [15] o o [16] GPIO 23       (OLED VDDC)
   (OLED VCC)    3.3V Power   [17] o o [18] GPIO 24
(Shared MOSI) MOSI / GPIO 10  [19] o o [20] Ground
 (Sensor SDO) MISO / GPIO 9   [21] o o [22] GPIO 25
(Shared SCLK) SCLK / GPIO 11  [23] o o [24] GPIO 8 / CE0  (Sensor CS)
                     Ground   [25] o o [26] GPIO 7 / CE1  (OLED CS)
                 EEPROM SDA   [27] o o [28] EEPROM SCL
                     GPIO 5   [29] o o [30] Ground
                     GPIO 6   [31] o o [32] GPIO 12
                    GPIO 13   [33] o o [34] Ground
                    GPIO 19   [35] o o [36] GPIO 16
                    GPIO 26   [37] o o [38] GPIO 20
                     Ground   [39] o o [40] GPIO 21
```
