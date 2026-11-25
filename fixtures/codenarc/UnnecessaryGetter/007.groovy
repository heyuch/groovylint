
            class Spec extends Specification {

                private final field = 'field'

                def test() {
                    expect:
                    field == 'field'
                    getField() == 'getField'
                    isField() == 'getField'
                }

                private static String getField() {
                    'getField'
                }
            }
            