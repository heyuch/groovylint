
            list.each { name -> doStuff()}
            shouldFail(Exception) { doStuff()}
            def c = { println 123}
            def m = [a:123, b: { println 7}]
        