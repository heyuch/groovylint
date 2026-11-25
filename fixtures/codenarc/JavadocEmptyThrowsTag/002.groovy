
        class MyClass {
            /**
             * Return the calculated count of some stuff.
             *
             * @param startIndex - the starting index
             * @throws
             * @throws RuntimeException if crazy sh*t happens
             *
             * NOTE: Only the first occurrence of an empty @return tag is found.
             *       So the following line is not flagged as a violation!!!
             * @throws
             */
            int countThings(int startIndex) { }

            /**
             *@throws
             */
            String otherMethod() { }
        }
        