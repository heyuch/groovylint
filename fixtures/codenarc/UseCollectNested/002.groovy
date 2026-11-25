
            def list = [1, 2, [3, 4, 5, 6], [7]]

            println list.collect { elem ->
                if (elem instanceof List)
                    elem.collect {it *2} // violation
                else elem * 2
            }

            println list.collect([8]) {
                if (it instanceof List)
                    it.collect {it *2} // violation
                else it * 2
            }
        