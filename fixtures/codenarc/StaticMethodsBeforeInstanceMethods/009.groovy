
            class MyClass {
                // Getters and Setters
                private String getPrivateStringValue1() { }
                private void setPrivateStringValue1(String value) { }
                protected String getStringValue2() { }
                protected void setStringValue2(String value) { }

                // Public
                public static int staticMethod1() { }
                public String getStringValue1() { }
                public void setStringValue1(String value) { }
                static final String staticMethod2(int id) { }
                public String method1() { }
                int method2() { }

                // Protected
                protected static staticMethod3() { }
                protected String method3() { }

                // Private
                private static staticMethod4() { }
                private int method4() { }
                int method5() { }
            }
        