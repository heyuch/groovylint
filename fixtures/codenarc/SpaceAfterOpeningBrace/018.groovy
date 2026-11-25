
            list.each {name -> }
            shouldFail(Exception) {doStuff() }
            def c = {println 123 }
            def m = [a:123, b: {println 7 }]
        