
            def x = !ready ? process('abc') : process('abc')
            def y = !ready ? increment(x) : increment(x)
        