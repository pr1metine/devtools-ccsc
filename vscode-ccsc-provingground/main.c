#include <16F883.h>

#fuses XT, NOWDT
#use delay(clock=4Mhz)

#include "sth/add.h"

void main() {
	output_low(PIN_C0);
	
	int c = add(4, 4);
	for (;;) {
		output_toggle(PIN_C0);
		delay_ms(500);
	}
}