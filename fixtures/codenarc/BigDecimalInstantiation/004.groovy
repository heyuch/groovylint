
            class MyClass {
                def myMethod() {
                    def b1 = new BigDecimal("123.45" as char[])
                    def b2 = new BigDecimal("123.45" as char[], 0, 6)
                    def b3 = new BigDecimal("123.45" as char[], 0, 3, MathContext.UNLIMITED)
                    def b4 = new BigDecimal("123.45" as char[], MathContext.UNLIMITED)
                }
            }