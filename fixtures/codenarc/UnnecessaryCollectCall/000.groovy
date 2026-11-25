
                // OK, is not method call in closure
                [1, 2, 3].collect { it * it }

                // OK, is not collect call
                [1, 2, 3].mapMethod { it.multiply(5) }

                // OK, is not one argument/closure call
                [1, 2, 3].collect(5)

                // OK is not single statement closure
                [1, 2, 3].collect { println it; it.multiply(5) }

                // OK is not single parameter closure
                [1, 2, 3].collect { a, b -> a.multiply(b) }

                // OK method call references closure parameters
                [1, 2, 3].collect { it.multiply(it) }

                // OK, chained methods, too complex to analyze (maybe implement later?)
                [1, 2, 3].collect { it.multiply(2).multiply(it) }
                ["1", "2", "3"].collect { it.bytes.foo(it) }
                [1, 2, 3].collect { it.multiply(2).multiply(4) }

                // should be written this way:
                [1, 2, 3]*.multiply(2)
                ["1", "2", "3"]*.bytes
        