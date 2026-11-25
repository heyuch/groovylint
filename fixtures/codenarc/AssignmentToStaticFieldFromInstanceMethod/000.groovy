
            class MyClass {
                private field1
                protected String field2 = 'abc'
                public int field3 = 123
                public final field4 = 456
                String property1 = 'abc'

                private void doStuff() {
                    field1 = new Object()
                    field2 = 'xxx'
                    field3 = 999
                    property1 = 'xxx'
                }
            }
        