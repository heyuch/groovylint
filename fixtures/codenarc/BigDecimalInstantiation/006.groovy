
            class MyClass {
                def myMethod() {
                    def b1 = new BigDecimal(99L)
                    final L = 12345 as long
                    def b2 = new BigDecimal(L, MathContext.UNLIMITED)
                }
            }