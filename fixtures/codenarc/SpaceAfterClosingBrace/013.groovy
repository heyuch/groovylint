
            list.each { name -> doStuff() }//comment
            shouldFail(Exception) { doStuff() }//comment
            def c = { println 123 }//comment
            def m = [a:123, b:{ println 7 }]            //not a violation
            def m2 = [a:123, b: m.each{ println it }]   //not a violation
        