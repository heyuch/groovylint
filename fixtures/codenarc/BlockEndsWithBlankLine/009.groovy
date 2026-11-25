
            // If a Closure is within another expression and the closing brace is not followed by anything else on the same line

            // Closure within a Map
            def myService = [
                doOtherStuff:{ id ->

                }
            ] as MyService

            // Closure within a List
            def list = [
                123,
                { id ->

                }   // a comment does not matter
            ]
        