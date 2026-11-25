
            class ValueClass {
                def clone() {
                }
            }

            class ValueClass2 {
                @Override
                public Object clone() {
                }
            }

            class ValueClass3 {
                @Override
                protected Object clone() throws CloneNotSupportedException {
                }
            }
        