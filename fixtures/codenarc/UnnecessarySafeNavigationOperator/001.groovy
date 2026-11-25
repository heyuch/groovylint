
            def myMethod() {
                "abc"?.bytes
                [1,2]?.name
                [abc:123]?.name
                [:]?.name
                123?.class
                123.45?.class
                Boolean.FALSE?.class
                Boolean.TRUE?.class
                this?.class
                super?.class
                new Long(0)?.class
            }
        