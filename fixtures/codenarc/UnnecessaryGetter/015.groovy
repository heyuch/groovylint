
            def "some test method"() {
                def x = Spy(SomeClass) {
                        getMethod() >> {
                    }
                }

                def y = Spy(SomeClass, constructorArgs: [1, 2, 3]) {
                        getMethod() >> {
                    }
                }
            }
            