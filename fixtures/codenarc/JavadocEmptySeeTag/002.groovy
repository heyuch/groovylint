
        class MyClass {
            /**
             * Return the calculated count of some stuff.
             *
             * @param startIndex - the starting index
             * @see
             * @throws RuntimeException if crazy sh*t happens
             *
             * NOTE: Only the first occurrence of an empty @return tag is found.
             *       So the following line is not flagged as a violation!!!
             * @see
             */
            int countThings(int startIndex) { }

            /**
             *@see
             */
            String otherMethod() { }
        }
        