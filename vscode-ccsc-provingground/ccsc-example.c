#include <16F883.h>

#fuses XT, NOWDT
#use delay(clock=4Mhz)

void main() {
    output_low(PIN_C0);

    for (;;) {
        output_toggle(PIN_C0);
        delay_ms(500);
    }
}