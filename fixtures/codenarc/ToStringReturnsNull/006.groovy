
            class MyClass {
                @Override
                String toString() {
                    StringBuilder sb = new StringBuilder()
                    (1..10).each { int index ->
                        sb << getCharacter[index]
                    }
                    return sb.toString();
                }
            }
        