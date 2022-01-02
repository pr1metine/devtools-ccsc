#include <16F883.h>

#fuses XT, NOWDT
#use delay(clock=4Mhz)

#include "sth/add.c"

void main() {
	int c = add(4, 4);
	output_low(PIN_C0);

	int c = something;
	for (;;) {
		output_toggle(PIN_C0);
		delay_ms(500);
	}
}