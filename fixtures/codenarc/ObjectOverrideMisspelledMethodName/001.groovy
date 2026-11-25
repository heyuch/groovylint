
            boolean equal(Object o) {}
            boolean equal(int other) {}                 // ok; wrong param type
            boolean equal(Object o, int other) {}       // ok; too many params
        