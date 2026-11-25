
            def x
            x = !ready ? 1 : 0
            x = !ready ? true : 0
            x = !ready ? 1 : false
            x = !ready ? MY_TRUE : Boolean.FALSE
            x = !ready ? Boolean.TRUE : x+1
            x = !ready ? increment(y) : increment(z)
            x = !ready ? 99 : 98+1
            x = !ready ? MIN_VALUE : MAX_VALUE
            x = !ready ? MAX_VALUE + 1 : MAX_VALUE
        