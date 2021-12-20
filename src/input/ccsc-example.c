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

/*
(translation_unit
  (preproc_include
    (system_lib_string))
  (preproc_call
    (preproc_directive)
    (preproc_arg))
  (preproc_call
    (preproc_directive)
    (preproc_arg))
  (function_definition
    (primitive_type)
    (function_declarator
      (identifier)
      (parameter_list))
    (compound_statement
      (expression_statement
        (call_expression
          (identifier)
          (argument_list
            (identifier))))
      (for_statement
        (compound_statement
          (expression_statement
            (call_expression
              (identifier)
              (argument_list
                (identifier))))
          (expression_statement
            (call_expression
              (identifier)
              (argument_list
                (number_literal)))))))))

*/
