
            class MyClass {
                /**
                 * ( x
                 * ) x
                 */
                void someMethod() {
                }

                /*
                 * ( 123
                 * )
                 */
                void someOtherMethod() {
                }

                void method3() {
                    if( running) { }    // This should be the ONLY violation
                }
            }
        