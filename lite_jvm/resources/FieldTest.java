public class FieldTest {
    public static final String NAME="static";

    private String fieldString ="default";
    private int a = 1;
    private Integer b = null;

    private Long c = 1L;
    private double fieldDouble = 100d;
    private float fieldFloat = 50f;
    public static int anInt = 1;
    static {
        anInt = 2;
    }

    public static void main(String[] args) {
        System.out.println(NAME);
        FieldTest test = new FieldTest();
        test.fieldFloat = 300f;
        System.out.println(test.fieldDouble);
        System.out.println(test.fieldFloat);
    }


}
