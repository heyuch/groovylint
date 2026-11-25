
            List<int> l = [1,2,3]
            def s = "some string ${l.collect { i -> i * i }.toString()}"
        