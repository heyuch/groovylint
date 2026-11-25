
            def x = true
            def y = {
                if (x) { return y }
                println x
            }
            def y = {
                return
            }
            return x
        