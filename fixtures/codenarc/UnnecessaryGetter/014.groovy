
            def "some test method"() {
                product = Stub(Product) {
                    getId() >> 10
                }

                product2 = Stub(constructorArgs:[1, 2, 3], Product) {
                    getId() >> 10
                }
            }
            