
            def a = 1 + 1
            def f = 'Hello' +           // OK because of line break
                        'World'
            def h = 'Hello' + null      // OK because not a string
            def i = 'Hello' + method()  // OK because not a string
            def j = 'Hello' - "$World"  // OK because not +
        