
            def myClosure = { int a, String b ->
                println a
                def myInnerClosure = { int c ->
                    c = 39
                    a = 0
                }
                b = null
                return myInnerClosure
            }
        