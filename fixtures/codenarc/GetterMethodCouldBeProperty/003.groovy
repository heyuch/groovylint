
            interface IResult {
                int getIntValue()
            }

            class MyClass {
                private static final CONSTANT_RESULT = 5

                IResult operate(int parameter) {
                    return new IResult() {
                        @Override
                        int getIntValue() {
                            return CONSTANT_RESULT
                        }
                    }
                }
            }
            