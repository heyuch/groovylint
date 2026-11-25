
            def foo  = Factory.foo()
            def bar = [foo  : foo()]
            def bar2 = foo  ?: foo()
        