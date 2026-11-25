
            def list = [1, 2, [3, 4, 5, 6], [7]]

            println list.collect { elem ->
                if (elem instanceof List) [elem, elem * 2].collect {it * 2}
                else elem * 2
            }
        