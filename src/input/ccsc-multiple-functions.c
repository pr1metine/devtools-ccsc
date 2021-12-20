#include <16F883.h>

#fuses XT, NOWDT
#use delay(clock=4Mhz)

int add(int a, int b) {
	return a + b;
}

int multiply(int a, int b) {
    return a * b;
}

void main() {
    int something(hello);
    output_low(PIN_C0);
    for (;;) {
        output_toggle(PIN_C0);
        delay_ms(500);
    }
}
