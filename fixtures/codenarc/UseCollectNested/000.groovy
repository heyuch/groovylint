
            // The collect() call is just one part of a larger expression; the result may be transformed into a different type
            [['a', 'b', 'c'], ['d', 'e']].collect { 'x' + it.collect { it + 'y' } }
            
            // The result from the collect() call is the object of another method call; the result may be a different type
            [['a', 'b', 'c'], ['d', 'e']].collect { it.collect { it + 'y' }.join(' ') }
        