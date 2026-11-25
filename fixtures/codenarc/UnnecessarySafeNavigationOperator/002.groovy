
            def myMethod() {
                "abc"?.toString()
                [1,2]?.toString()
                [abc:123]?.toString()
                [:]?.toString()
                123?.getClass()
                123.45?.getClass()
                Boolean.FALSE?.getClass()
                Boolean.TRUE?.getClass()
                this?.getClass()
                super?.getClass()
                new Long(100)?.toString()
            }
        