
            class SomeClass implements Serializable {}
            def main() {
                def (int i, String j, SomeClass s, dynamic, List l) = [10, 'foo', new SomeClass(), 42, [1,2,3]]
            }
        