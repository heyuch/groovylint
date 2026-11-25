
            def foo() {}
            def foo(int x) {}
            def bar() {}
            def bar(int x) {}
            def baz = {}
            def bif = {}
            def car = 100
            class MyClass {
                def foo() {}
                def foo(int x) {}
                def bar() {}
                def bar(int x) {}
                def baz = {}
                def bif = {}
                def car = 100
                def x = new Object() {
                    def foo() {}
                    def foo(int x) {}
                    def y = new Object() {
                        def foo() {}
                        def foo(int x) {}
                    }
                }
            }
        