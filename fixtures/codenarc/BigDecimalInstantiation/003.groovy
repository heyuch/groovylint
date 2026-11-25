
            class MyClass {
                def myMethod() {
                    def b1 = new BigDecimal(BigInteger.ONE)
                    def b2 = new BigDecimal(BigInteger.TEN, 3)
                    def b3 = new BigDecimal(BigInteger.TEN, 3, MathContext.UNLIMITED)
                    def b4 = new BigDecimal(BigInteger.TEN, MathContext.UNLIMITED)
                }
            }