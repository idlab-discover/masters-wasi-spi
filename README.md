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
