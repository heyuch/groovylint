
            final uninitialized_variable
            def x = 1              // ok
            def z = null
            def (f, g) = [1, 2]    // ok
            (a, b, c) = [1, 2, 3]  // ok
            class MyClass {
                def a = null, b = null, c = null
                def d, e, f
            }
        